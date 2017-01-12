use std::collections::HashMap;
use std::io::Write;
use std::os::raw::c_uchar;
use std::process;
use mustache;
use mustache::MapBuilder;

use client::ClientW;
use config::Config;
use util;
use workspace::Workspace;

pub struct LoggerConfig {
    client_template: mustache::Template,
    client_selected_template: mustache::Template,
    client_title_length: usize,
    separator: &'static str,
    tag_template: mustache::Template,
    tag_selected_template: mustache::Template,
}

impl LoggerConfig {
    pub fn client_template(mut self, template: &str) -> Self {
        self.client_template = mustache::compile_str(template).unwrap();
        self
    }

    pub fn client_selected_template(mut self, template: &str) -> Self {
        self.client_selected_template = mustache::compile_str(template).unwrap();
        self
    }

    pub fn client_title_length(mut self, len: usize) -> Self {
        self.client_title_length = len;
        self
    }

    pub fn separator(mut self, s: &'static str) -> Self {
        self.separator = s;
        self
    }

    pub fn tag_template(mut self, template: &str) -> Self {
        self.tag_template = mustache::compile_str(template).unwrap();
        self
    }

    pub fn tag_selected_template(mut self, template: &str) -> Self {
        self.tag_selected_template = mustache::compile_str(template).unwrap();
        self
    }
}

impl Default for LoggerConfig {
    fn default() -> Self {
        LoggerConfig {
            client_template: mustache::compile_str("[{{& content }}] ").unwrap(),
            client_selected_template: mustache::compile_str("[<fc=#FFFF00>{{& content }}</fc>] ")
                .unwrap(),
            client_title_length: 8,
            separator: " :: ",
            tag_template: mustache::compile_str("{{& content }}").unwrap(),
            tag_selected_template: mustache::compile_str("<fc=#00FF00>{{& content }}</fc> |")
                .unwrap(),
        }
    }
}

pub trait Logger {
    fn dump(&mut self,
            global_config: &Config,
            workspaces: &HashMap<c_uchar, Workspace>,
            clients: &Vec<ClientW>,
            current_tag: c_uchar,
            current_clients: &Vec<ClientW>,
            focused: Option<ClientW>);
}

pub struct DummyLogger;

impl DummyLogger {
    #[allow(unused_variables)]
    pub fn new(config: LoggerConfig) -> Self {
        DummyLogger
    }
}

impl Logger for DummyLogger {
    #[allow(unused_variables)]
    fn dump(&mut self,
            global_config: &Config,
            workspaces: &HashMap<c_uchar, Workspace>,
            clients: &Vec<ClientW>,
            current_tag: c_uchar,
            current_clients: &Vec<ClientW>,
            focused: Option<ClientW>) {
        // Do nothing.
    }
}

pub struct XMobarLogger {
    config: LoggerConfig,
    child_stdin: process::ChildStdin,
}

impl XMobarLogger {
    pub fn new(config: LoggerConfig, xmobar_args: &[&str]) -> Self {
        let process::Child { stdin: child_stdin, .. } = process::Command::new("xmobar")
            .stdin(process::Stdio::piped())
            .args(xmobar_args)
            .spawn()
            .expect("cannot spawn xmobar");
        XMobarLogger {
            config: config,
            child_stdin: child_stdin.unwrap(),
        }
    }
}

impl Logger for XMobarLogger {
    fn dump(&mut self,
            global_config: &Config,
            workspaces: &HashMap<c_uchar, Workspace>,
            clients: &Vec<ClientW>,
            current_tag: c_uchar,
            current_clients: &Vec<ClientW>,
            focused: Option<ClientW>) {
        fn render<W: Write, T: Into<String>>(w: &mut W,
                                             template: &mustache::Template,
                                             to_render: T) {
            let content =
                mustache::MapBuilder::new().insert_str("content", to_render.into()).build();
            template.render_data(w, &content).unwrap();
        }

        let mut tags: Vec<char> = clients.iter().map(|c| c.tag() as char).collect();
        tags.push(current_tag as char);
        tags.sort();
        tags.dedup();
        for t in &tags {
            let selected_template = if *t == current_tag as char {
                &self.config.tag_selected_template
            } else {
                &self.config.tag_template
            };
            if current_tag == 0 {
                render(&mut self.child_stdin, selected_template, "Overview");
            } else {
                let string = if let Some(description) =
                    workspaces.get(&(*t as c_uchar)).unwrap().get_description() {
                    format!("{} - {}", t, description)
                } else {
                    (*t).to_string()
                };
                render(&mut self.child_stdin, selected_template, string);
            }
        }
        write!(self.child_stdin, "{}", self.config.separator);
        for i in 0..current_clients.len() {
            let c = &current_clients[i];
            let selected_template = match focused.as_ref() {
                Some(c_focused) if c_focused.window() == c.window() => {
                    &self.config.client_selected_template
                }
                _ => &self.config.client_template,
            };
            let msg = if current_tag == 0 {
                format!("{}@", c.tag() as char)
            } else {
                "".to_string()
            };
            render(&mut self.child_stdin,
                   selected_template,
                   format!("<{}> {}{} ",
                           i + 1,
                           msg,
                           util::truncate(&c.get_title(), self.config.client_title_length)));
        }
        write!(self.child_stdin, "\n");
    }
}
