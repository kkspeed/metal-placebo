use std::collections::HashMap;
use std::io::Write;
use std::os::raw::c_uchar;
use std::process;

use client::ClientW;
use config::Config;
use workspace::Workspace;

pub struct LoggerConfig {
    pub client_color: &'static str,
    pub client_selected_color: &'static str,
    pub separator_color: &'static str,
    pub tag_color: &'static str,
    pub tag_selected_color: &'static str,
}

impl LoggerConfig {
    pub fn client_color(mut self, color: &'static str) -> Self {
        self.client_color = color;
        self
    }

    pub fn client_selected_color(mut self, color: &'static str) -> Self {
        self.client_selected_color = color;
        self
    }

    pub fn separator_color(mut self, color: &'static str) -> Self {
        self.separator_color = color;
        self
    }

    pub fn tag_color(mut self, color: &'static str) -> Self {
        self.tag_color = color;
        self
    }

    pub fn tag_selected_color(mut self, color: &'static str) -> Self {
        self.tag_selected_color = color;
        self
    }
}

impl Default for LoggerConfig {
    fn default() -> Self {
        LoggerConfig {
            client_color: "#FFFFFF",
            client_selected_color: "#FFFF00",
            separator_color: "#000000",
            tag_color: "#FFFFFF",
            tag_selected_color: "#00FF00",
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
    pub fn new(config: LoggerConfig, xmobar_args: &[&str]) -> XMobarLogger {
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
        let mut tags: Vec<char> = clients.iter().map(|c| c.tag() as char).collect();
        tags.push(current_tag as char);
        tags.sort();
        tags.dedup();
        let mut result = String::new();
        for t in &tags {
            if *t == current_tag as char {
                if current_tag == 0 {
                    result += &format!("<fc={}> Overview </fc>|", self.config.tag_selected_color);
                } else {
                    result += &if let Some(description) =
                        workspaces.get(&(*t as c_uchar)).unwrap().get_description() {
                        format!("<fc={}> {} - {} </fc>|",
                                self.config.tag_selected_color,
                                t,
                                description)
                    } else {
                        format!("<fc={}> {} </fc>|", self.config.tag_selected_color, t)
                    };
                }
            } else {
                result += &if let Some(description) =
                    workspaces.get(&(*t as c_uchar)).unwrap().get_description() {
                    format!("<fc={}> {} - {} </fc>|",
                            self.config.tag_color,
                            t,
                            description)
                } else {
                    format!("<fc={}> {} </fc>|", self.config.tag_color, t)
                };
            }
        }

        result += " :: ";
        for c in current_clients {
            let color = match &focused {
                &Some(ref c_focused) if c_focused.window() == c.window() => {
                    self.config.client_selected_color
                }
                _ => self.config.client_color,
            };
            let msg = if current_tag == 0 {
                format!("{}@", c.tag() as char)
            } else {
                "".to_string()
            };
            result += &format!("[<fc={}>{1:.8}</fc>] ", color, msg + &c.get_title());
        }
        writeln!(self.child_stdin, "{}", result).unwrap();
    }
}
