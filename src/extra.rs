use core::WindowManager;
use std::rc::Rc;

use prompt;
use prompt::Prompt;

pub fn add_window_user_tag_dmenu(w: &mut WindowManager) {
    if let Some(c) = w.current_focused() {
        let hint = format!("add uesr tag to {}: ", c.get_class());
        let args = vec!["-p", &hint];
        let selections = vec![];
        let prompt = prompt::DmenuPrompt::new(&selections, &args);
        match prompt.do_prompt().as_ref() {
            Ok(result) if !result.trim().is_empty() => {
                c.clone().put_extra("user_tag".to_string(), result.trim().to_string())
            }
            _ => return,
        }
    }
}

pub fn add_workspace_user_tag_dmenu(w: &mut WindowManager) {
    let selections = vec![];
    let args = vec!["-p", "tag: "];
    let prompt = prompt::DmenuPrompt::new(&selections, &args);
    match prompt.do_prompt().as_ref() {
        Ok(result) if !result.trim().is_empty() => {
            w.current_workspace_mut().set_description(result.trim())
        }
        _ => return,
    }
}

pub fn select_window_dmenu(w: &mut WindowManager) {
    let clients = w.all_clients();
    let contents: Vec<String> = clients.iter()
        .map(|c| {
            format!("[({}) {}] {}",
                    c.get_extra("user_tag").unwrap_or(Rc::new("".to_string())),
                    c.get_class(),
                    c.get_title())
        })
        .collect();
    let args = vec!["-p",
                    "window",
                    "-i",
                    "-l",
                    "7",
                    "-sb",
                    "#000000",
                    "-sf",
                    "#00ff00",
                    "-nb",
                    "#000000",
                    "-nf",
                    "#dddddd",
                    "-fn",
                    "WenQuanYi Micro Hei Mono-12"];
    let prompt = prompt::DmenuPrompt::new(&contents, &args);
    match prompt.do_prompt() {
        Ok(result) => {
            if let Some(position) = contents.iter().position(|s| (*s).trim() == result.trim()) {
                let c = clients[position].clone();
                w.select_tag(c.tag());
                w.set_focus(c);
            }
        }
        Err(_) => return,
    }
}